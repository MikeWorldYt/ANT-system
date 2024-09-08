import { Component, OnInit } from '@angular/core';
import { FooterComponent } from '../../../layout/footer/footer.component';

// ▲ SERVICES ▲
import { LanguageService } from '../../../services/navLanguage.service';
import { TitleService } from '../../../services/navTitle.service';
import { PageService } from '../../../services/navPage.service';
import { ArticleService } from '../../../services/navArticleObserver.service';

// ▲ CONTENT ▲
import { content } from '../../content/content';

@Component({
  selector: 'docs-t03-into-nomenclature',
  standalone: true,
  imports: [
    FooterComponent
  ],
  templateUrl: './into-main-getting.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T03_IntoComponent implements OnInit {
  // ▲ SERVICES ▲
  constructor(
    private languageService: LanguageService,
    private titleService: TitleService,
    private pageService: PageService,
    private intersectionService: ArticleService,
  ) { }

  // ▬▬▬ For inner content
  write: any;

  // ▬▬▬ Navigation Context
  currentLanguage: string = 'EN';
  currentTitle: string = 'title_03';
  currentPage: string = 'page_01';
  currentArticle: string = '';

  // ███ OnInit ███ 
  ngOnInit(): void {
    this.titleService.setTitle('title_03');
    this.pageService.setCurrentPage('page_01');
  }


}
