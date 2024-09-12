import { Component, input } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { CommonModule } from '@angular/common';
// ▲ COMPONENTS ▲
import { FooterComponent } from '../../../layout/footer/footer.component';

// ▲ SERVICES ▲
import { TitleService } from '../../../services/navTitle.service';
import { PageService } from '../../../services/navPage.service';

// ▲ FEATURES ▲
import { ContentPageComponent } from '../../content-page/content-page.component';

@Component({
  selector: 'docs-t01-into-getting-starter',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    FooterComponent,
    ContentPageComponent
  ],
  templateUrl: './into-getting-starter.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T01_IntoComponent {
  // @Input() 

  // ▲ SERVICES ▲
  constructor(
    private TitleService: TitleService,
    private pageService: PageService,
  ) { }

  // ▬▬▬ Navigation Context
  currentLanguage: string = 'EN';
  currentTitle: string = 'title_01';
  currentPage: string = 'page_01';

  // ███ OnInit ███
  ngOnInit(): void {
    this.TitleService.setTitle(this.currentTitle);
    this.pageService.setCurrentPage(this.currentPage);
  }

}