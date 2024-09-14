import { Component, Input } from '@angular/core';

// ▲ SERVICES ▲
import { LanguageService } from '../../services/navLanguage.service';
import { TitleService } from '../../services/navTitle.service';
import { PageService } from '../../services/navPage.service';

// ▲ FEATURES ▲
import { ContentPageComponent } from '../content-page/content-page.component';

@Component({
  selector: 'app-page',
  standalone: true,
  imports: [
    ContentPageComponent,
  ],
  templateUrl: './page.component.html',
  styleUrl: './page.component.css'
})
export class DocsPageComponent {
  @Input() set lang(newLang: string){
    this.validLang = (newLang === 'EN' || newLang === 'ES') ? newLang : 'EN';
    this.language.setLanguage(this.validLang);
    this.updatePath();
  };
  @Input() set title (newTitle: string) { 
    this.currentTitle = newTitle;
    this.titleService.setTitle(newTitle);
    this.updatePath();
  };
  @Input() set page(newPage: string) {
    this.currentPage = newPage;
    this.pageService.setCurrentPage(newPage);
    this.updatePath();
  }

  constructor(
    private language: LanguageService,
    private titleService: TitleService,
    private pageService: PageService,
  ) { }

  // ▬▬▬ Navigation Context
  validLang: string = '';
  currentTitle: string = '';
  currentPage: string = '';
  path: [string, string, string] | null = null;

  private updatePath(): void {
    this.path = null; // When the path is updated, it forces a re-render
    setTimeout(() => {
      this.path = [this.validLang, this.currentTitle, this.currentPage];
    }, 0);
  }

}